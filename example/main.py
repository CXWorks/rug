import subprocess
import subprocess as sp
import os
import copy
import re
import json
import sys
from openai import OpenAI
import tiktoken
import time
import multiprocessing

init_content = """
You are an expert in Rust and I need your help on development. I will provide you the context and definition or sample 
and will ask you to help me write the code. Please pay attention to the paths and try to utilize the information I provided.
"""

counter = 0
msgs = [
    {"role": "system", "content": init_content},

]

memo = {}


def gpt_request(messages: list):
    global client
    finished = False
    msg = ''
    while not finished:
        try:
            response = client.chat.completions.create(
                model="gpt-3.5-turbo",
                # model= "gpt-4-1106-preview",
                presence_penalty=-1,
                frequency_penalty=0.5,
                messages=messages,
            )
            msg = response.choices[0].message.content
            print("=" * 40)
            print(messages[-1]['content'])
            print('-' * 20)
            print(msg)
            time.sleep(1)
            finished = True
        except Exception as e:
            print('err', e)
            if "This model's maximum context length is " in str(e):
                break
            if "Connection err" in str(e):
                client = OpenAI(api_key='')
            time.sleep(15)
    return (finished, msg)


def init_global(data):
    global single_path_import, glob_path_import, client
    client = OpenAI(api_key='')
    glob = data['glob_path_import']
    glob_path_import = []
    glob_path_import = [(x, glob[x]) for x in glob]
    glob_path_import.sort(key=lambda x: len(x[0]), reverse=True)
    single_path_import = data['single_path_import']


def parse_log(file: str):
    ans = {}

    if not os.path.exists(file):
        print(file)
        raise Exception()
    with open(file, 'r') as fp:
        ls = fp.readlines()
        i = 0
        while i < len(ls):
            if ls[i].startswith("----"):
                lifetimes = ''
                idx = ls[i + 1].find(' ')
                target_file = ls[i + 1][:idx]
                target_func = ls[i + 1][idx + 1:-1]
                i += 1
                if ls[i + 1].startswith('\''):
                    lifetimes = ls[i + 1][:-1]
                    i += 1
                if not ls[i + 1].startswith("deps:"):
                    print(ls[i])
                    print(ls[i+1])
                assert ls[i + 1].startswith("deps:")
                deps = json.loads(ls[i + 1][5:])
                assert ls[i + 2].startswith("candidates:")
                candidates = json.loads(ls[i + 2][11:])
                i += 2
                stmts = []
                j = i + 1
                tys = []
                func_call = set()
                func_calls = []
                while j < len(ls) and not ls[j].startswith("-----"):
                    if ls[j].startswith('+'):
                        fc = ls[j][1:]
                        if fc not in func_call:
                            func_calls.append(fc)
                            func_call.add(fc)
                    else:
                        if '//' not in ls[j]:
                            pass
                        else:
                            stmts.append(ls[j])
                            if 'None+' in ls[j]:
                                tys.append((None, ls[j].split('+')[1].strip()))
                            else:
                                tys.append((ls[j].split('//')[1].strip(), None))
                    j += 1
                i = j
                if target_file not in ans:
                    ans[target_file] = []
                ans[target_file].append((target_file, stmts, func_calls, lifetimes, deps, candidates, target_func, tys))
            else:
                i += 1
    return ans


def is_std(s: str):
    if s.startswith("&mut") or s.startswith("& mut "):
        s = s[s.find("mut") + 4:]
    elif s.startswith("&"):
        s = s[max(s.find(" ") + 1, 1):]
    if s.startswith('std::') or s.startswith('core::') or s.startswith('alloc::'):
        return True
    else:
        return False


def handle_gpt_output(code:str):
    ans = []
    in_it = False
    if '```' not in code:
        return code
    for l in code.splitlines():
        if not in_it:
            if '```rust' in l or '```Rust' in l or '```RUST' in l:
                in_it = True
        else:
            if '```' in l:
                in_it = False
            else:
                ans.append(l)
    return "\n".join(ans)

def load_analysis(f: str):
    ans = None
    with open(f, 'r') as fp:
        ans = json.load(fp)
    return ans


def prompt_with_bounds(parent_def_path, def_path, ty, bounds, cans, deps, candidates, data, crate, file, src_pq, fd,
                       recur):
    if 'RUG_ANY' in cans and len(cans) > 1:
        cans = [x for x in filter(lambda x: x != 'RUG_ANY', cans)]
    found = False
    concrete_can = {}
    std_count = 0
    has_succeed = False
    for can in filter(lambda x: not (
            (x.startswith('<') or '::<' in x) and x.endswith('>')) and x != 'std::io::Stdin' and x not in recur,
                      cans):
        found = True
        recur.add(can)
        if is_std(can):
            std_count += 1
            if std_count < 3:
                concrete_can[can] = prompt_built_in(fd, parent_def_path, can, file, crate)
        elif can == 'RUG_ANY':
            concrete_can[can] = prompt_built_any(parent_def_path, def_path, ty)
        else:
            if len(deps.get(can, [])) == 0:
                # no other depends
                # prompt directly
                concrete_can[can] = prompt_with_src_only(parent_def_path, ty, can, data, fd, src_pq, file, crate)
            else:
                pass
                # concrete_can[can] = prompt_pre_context(parent_def_path, def_path, can, data, crate, file, src_pq)
                map = {}
                for k, vs in deps[can].items():
                    map[k] = prompt_with_bounds(can, can, k, vs, candidates.get(can, {}).get(k, []), deps, candidates,
                                                data, crate, file, src_pq, fd, recur)
                concrete_can[can] = prompt_with_context(parent_def_path, def_path, can, data, fd, file, map, src_pq, crate)
        recur.remove(can)
    if found:
        prompt = "For `{}` type in `{}`, we have {} candidates: `{}`\n".format(get_full_path(ty),
                                                                               get_full_path(parent_def_path),
                                                                               len(concrete_can), "`, `".join(
                [get_full_path(x) for x in concrete_can.keys()]))
        if len(concrete_can) == 1 and 'RUG_ANY' in concrete_can:
            prompt = "For `{}` type in `{}`, we don't find explicit bounds.\n".format(get_full_path(ty),
                                                                                      get_full_path(parent_def_path))
        for can, v in concrete_can.items():
            prompt += v[2] + "\n"
            src_pq.append(can)
        for can, v in concrete_can.items():
            if v[0]:
                return v
        return (False, '', prompt, ty)
    if not found:
        for can in filter(lambda x: (x.startswith('<') or '::<' in x) and x.endswith('>') and x not in recur, cans):
            # assert len(deps.get(can, {})) == 1
            for nty, nbounds in deps.get(can, {}).items():
                recur.add(can)
                found = True
                src_pq.append(nty)
                src_pq.append(can)
                ans = prompt_with_bounds(
                    can, can, nty, nbounds, candidates.get(can, {}).get(nty, []), deps, candidates, data, crate, file,
                    src_pq, fd, recur)
                prompt = "For `{}` type in `{}`, `{}` can be used: \n".format(get_full_path(ty),
                                                                              get_full_path(parent_def_path),
                                                                              get_full_path(can)) + ans[2]
                recur.remove(can)
                return (ans[0], ans[1], prompt, ans[3])
    if not found:
        for x in bounds:
            src_pq.append(x)
        return (False, '',
                "For `{}` type in `{}`, you need to write a concrete implementation that satisfied bounds: `{}`.\n".format(
                    get_full_path(ty), get_full_path(parent_def_path), ", ".join([get_full_path(x) for x in bounds])),
                ty)
    assert False


def prompt_built_any(parent_def_path, def_path, ty):
    if ty in memo:
        print('cached', ty)
        return memo[ty]
    prompt = "The `{}` in `{}` doesn't have type bounds. It might have other implicit bounds".format(get_full_path(ty),
                                                                                                     get_full_path(
                                                                                                         def_path))
    memo[ty] = (False, '', prompt, ty)
    return (False, '', prompt, ty)


def prompt_built_in(fd, parent_def_path, ty, file, crate):
    if ty in memo:
        print('cached', ty)
        return memo[ty]
    prompt = "the `{}` can be used in {}. ".format(get_full_path(ty), get_full_path(parent_def_path))
    messages = copy.deepcopy(msgs)
    global counter
    counter += 1
    var_name = 'v' + str(counter)
    content = """Please help me fill in the following code by creating an initialized local variable named `{}` with type `{}` using its constructor method or structual build in `{}` crate's {} file.
    Fill in any sample data if necessary. The code to fill is below and don't change function and mod names. Pay attention to the paths and reply the whole mod code only without other explanantions.
```rust
#[cfg(test)]
mod tests_prepare {{
    #[test]
    fn sample() {{
        let mut {} = // create the local variable {} with type {}
    }}
}}
```"""
    messages.append(
        {"role": "user", "content": content.format(var_name, get_full_path(ty), crate, file, var_name, var_name, get_full_path(ty))}
    )
    finished = False
    count = 3
    while not finished and count > 0:
        has_ans, code = gpt_request(messages)
        code = handle_gpt_output(code)
        if has_ans and compile_verify(fd, file, code, 'tests_prepare', var_name, ty, crate):
            finished = True
        else:
            count -= 1
    memo[ty] = (finished, code, prompt, ty)
    return (finished, code, prompt, ty)


def compile_only(fd, file, code, crate):
    ans = False
    with open(fd + '/' + file, 'r+') as fp:
        origins = fp.readlines()
        mutate = copy.deepcopy(origins)
        code = code.replace("use {}::".format(crate.replace('-', '_')), "use crate::")
        print('$'*40)
        print(code)
        print('*'*40)
        mutate.append(code)
        fp.truncate(0)
        fp.seek(0)
        fp.writelines(mutate)
        fp.flush()
        os.fsync(fp.fileno())
        ret = subprocess.run("cargo test -- --list", shell=True, cwd=fd, capture_output=True)
        if ret.returncode == 0:
            ans = True
        else:
            print(ret.stderr.decode('utf-8'))
            fp.truncate(0)
            fp.seek(0)
            fp.writelines(origins)
            fp.flush()
            os.fsync(fp.fileno())

    return ans


def compile_verify(fd, file, code, mod, var_name, ty, crate):
    ans = False
    with open(fd + '/' + file, 'r+') as fp:
        origins = fp.readlines()
        mutate = copy.deepcopy(origins)
        code = code.replace("use {}::".format(crate.replace('-', '_')), "use crate::")
        mutate.append(code)
        fp.truncate(0)
        fp.seek(0)
        fp.writelines(mutate)
        fp.flush()
        os.fsync(fp.fileno())
        ret = subprocess.run("cargo clean && RUG_VERIFY=1 MOD={} VAR={} cargorunner rudra".format(mod, var_name),
                             shell=True, cwd=fd, capture_output=True)
        if ret.returncode == 0:
            for l in ret.stdout.decode('utf-8').splitlines():
                print('compare', ty.strip(), l.strip(), 'res is', ty.strip() == l.strip() or ty.strip() in l)
            ans = True
        else:
            print(ret.stderr.decode('utf-8'))
        fp.truncate(0)
        fp.seek(0)
        fp.writelines(origins)
        fp.flush()
        os.fsync(fp.fileno())
    return ans


def get_real_path(s: str):
    return s[s.find("\"") + 1:s.rfind("\"")]


def prompt_with_context(parent_def_path, def_path, ty, data, fd, file, ctxt, src_pq, crate):
    if ty in memo:
        print('cached', ty)
        return memo[ty]
    global counter
    counter += 1
    var_name = 'v' + str(counter)
    targets = data['targets']
    dependencies = data['dependencies']
    srcs = data['srcs']
    struct_to_trait = data['struct_to_trait']
    trait_to_struct = data['trait_to_struct']
    self_to_fn = data['self_to_fn']
    type_to_def_path = data['type_to_def_path']
    struct_constructor = data['struct_constructor']
    prompt = "for `{}` used as `{}`, ".format(get_full_path(ty), get_full_path(def_path))
    cons = [get_full_path(x) for x in filter(lambda x: x not in ['clone'], struct_constructor.get(ty, []))]
    constructors = ''
    if len(cons) > 0:
        prompt += "try to use constructor functions like `{}` to build `{}`. ".format(", ".join(cons),
                                                                                      get_full_path(ty))
        constructors = "try to use constructor functions like `{}` to build `{}`. ".format(", ".join(cons),
                                                                                           get_full_path(ty))

    src_pq.append(ty)

    code = ''
    file_loc = ''
    if ty in srcs:
        code += srcs[ty][0]
        file_loc = " in {}".format(get_real_path(srcs[ty][1]))
    if ty in self_to_fn:
        for c in self_to_fn[ty]:
            if c not in 'CloneCopyDebug':
                code += c + '\n'
    info = prompt_struct.format(get_full_path(ty), file_loc, code)
    # context info
    for k, v in ctxt.items():
        if v[0]:
            # has concrete example
            tp = 'For the generic arg `{}`, `{}` can be used, the code to construct it as a local variable is shown below and is verified. Please reuse it without modifcations of statements.\n```rust\n{}\n```'.format(
                k, get_full_path(v[3]), v[1])
        else:
            tp = 'For the generic arg `{}`, here are the hints: {}'.format(k, v[2])
        info += tp
        prompt += tp
    messages = copy.deepcopy(msgs)
    content = """Please help me fill in the following code by creating an initialized local variable named `{}` with type `{}` using its constructor method or structual build. {}
{}  
The code to fill is below and don't change function and mod names. Fill in any sample data if necessary. Pay attention to the paths and reply with the code only without other explanantions.
```rust
#[cfg(test)]
mod tests_prepare {{
    #[test]
    fn sample() {{
        let mut {} = // create the local variable {} with type {}
    }}
}}
```"""
    messages.append(
        {"role": "user", "content": content.format(var_name, get_full_path(ty), constructors, info, var_name, var_name,
                                                   get_full_path(ty))}
    )
    finished = False
    count = 3
    while not finished and count > 0:
        has_ans, code = gpt_request(messages)
        code = handle_gpt_output(code)
        if has_ans and compile_verify(fd, file, code, 'tests_prepare', var_name, ty, crate):
            finished = True
        else:
            count -= 1
    memo[ty] = (finished, code, prompt, ty)
    return (finished, code, prompt, ty)


def prompt_with_src_only(parent_def_path, def_path, ty, data, fd, src_pq, file, crate):
    if ty in memo:
        return memo[ty]
    src_pq.append(ty)
    targets = data['targets']
    dependencies = data['dependencies']
    srcs = data['srcs']
    struct_to_trait = data['struct_to_trait']
    trait_to_struct = data['trait_to_struct']
    self_to_fn = data['self_to_fn']
    type_to_def_path = data['type_to_def_path']
    struct_constructor = data['struct_constructor']
    struct_constructor = data['struct_constructor']

    prompt = "the `{}` satisfies `{}` in `{}`. ".format(get_full_path(ty), get_full_path(def_path),
                                                        get_full_path(parent_def_path))
    cons = [get_full_path(x) for x in filter(lambda x: x not in ['clone'], struct_constructor.get(ty, []))]
    constructors = ''
    if len(cons) > 0:
        prompt += "Try to use constructor functions like `{}` to build `{}`. ".format(", ".join(cons),
                                                                                      get_full_path(ty))
        constructors = "Try to use constructor functions like `{}` to build `{}`. ".format(", ".join(cons),
                                                                                           get_full_path(ty))
    code = ''
    file_loc = ''
    if ty in srcs:
        code += srcs[ty][0]
        file_loc = " in {}".format(get_real_path(srcs[ty][1]))
    if ty in self_to_fn:
        for c in self_to_fn[ty]:
            if c not in 'CloneCopyDebug':
                code += c + '\n'
    info = prompt_struct.format(get_full_path(ty), file_loc, code)
    messages = copy.deepcopy(msgs)
    global counter
    counter += 1
    var_name = 'v' + str(counter)
    content = """Please help me fill in the following code by creating an initialized local variable named `{}` with type `{}` using its constructor method or structual build in `{}` crate {} file. {}
{}  
The code to fill is below and don't change function and mod names. Fill in any sample data if necessary. Pay attention to the paths and reply with the code only without other explanantions.
```rust
#[cfg(test)]
mod tests_prepare {{
    #[test]
    fn sample() {{
        let mut {} = // create the local variable {} with type {}
    }}
}}
```"""
    messages.append(
        {"role": "user", "content": content.format(var_name, get_full_path(ty), crate, file, constructors, info, var_name, var_name,
                                                   get_full_path(ty))}
    )
    finished = False
    count = 3
    while not finished and count > 0:
        has_ans, code = gpt_request(messages)
        code = handle_gpt_output(code)
        if has_ans and compile_verify(fd, file, code, 'tests_prepare', var_name, ty, crate):
            finished = True
        else:
            count -= 1
    memo[ty] = (finished, code, prompt, ty)
    return (finished, code, prompt, ty)


def get_full_path(ty: str):
    if ty in single_path_import:
        return single_path_import[ty]
    for k, v in glob_path_import:
        if ty.startswith(k):
            t = ''
            if len(v) > 1:
                t = v
            assert not (t + ty[len(k) + 2:]).startswith("::")
            return t + ty[len(k) + 2:]
    return ty



prompt_target = """The target function is `{}` in `{}` crate's {} file, its definition path is `{}`{} and source code is like below:
```rust
{}
```

"""

prompt_dep = """The bounds and generic parameter info is shown below:
```
{}
```

"""

prompt_struct = """ The relevant definition, and method of `{}`{} are shown below:
```rust
{}
```
"""

prompt_impls = """The `{}` impls `{}` traits.
"""

prompt_rimpls = """The `{}` trait has `{}` that implements it.
"""

uid = 0


def run_single_fd(fd:str):
    global uid
    enc = tiktoken.encoding_for_model("gpt-4")
    import copy
    init = """You are an expert in Rust. I need your help to develop unit tests for the given function in the crate.I will give you the information about the target function and relevant definitions. I may give you the sample code to build the parameters, please strictly follow the sample code to construct the variable (you can change the variable names) and its use statements since these code are verified. Please only output the unit test(Rust code) for the targetfunction without any explainations and be strict about compiler checks and import paths. Please prepare the inital test data if necessary."""
    msgs = [
        {"role": "system", "content": init},

    ]
    ok = 0
    exceed_16 = 0
    exceed_128 = 0
    total = 0
    fin = subprocess.run('cargo ws list -l', shell=True, cwd=fd, capture_output=True)
    for l in fin.stdout.decode('utf-8').splitlines():
        ls = l.split(' ')
        crate = ls[0].strip()
        path = ls[-1]
        if not os.path.exists(fd+'/'+crate+'.json'):
            subprocess.run('cargo clean && CHAT_UNIT=1 cargorunner rudra', shell=True, capture_output=True, cwd=fd+'/'+path)
            subprocess.run('mv preprocess.json {}.json'.format(crate), shell=True, capture_output=True, cwd=fd)
        if not os.path.exists(fd+"/"+crate+".out.txt"):
            fin = subprocess.run('cargo clean && UNIT_GEN=s1 cargorunner rudra', shell=True, capture_output=True, cwd=fd+'/'+path)
            with open(fd+"/"+crate+".out.txt", 'w') as fp:
                fp.writelines("\n".join(fin.stdout.decode("utf-8").splitlines()))
        if os.path.exists(fd+'/'+crate+'.json'):
            data = load_analysis(fd+'/'+crate+'.json')
            targets = data['targets']
            dependencies = data['dependencies']
            srcs = data['srcs']
            struct_to_trait = data['struct_to_trait']
            trait_to_struct = data['trait_to_struct']
            self_to_fn = data['self_to_fn']
            type_to_def_path = data['type_to_def_path']
            init_global(data)
            ans = parse_log(fd + "/{}.out.txt".format(crate))
            for f, vv in ans.items():
                file = f
                for (_, stmts, func_call, lifetimes, deps, candidates, target_func, tys) in vv:
                    if file.startswith("/home") or target_func.endswith(">::fmt"):
                        continue
                    if target_func not in targets:
                        print('missing', target_func)
                        continue
                    uid += 1
                    total += 1
                    meta = targets[target_func]
                    func_name = meta[0]
                    optional_trait = meta[2]
                    final_prompt = ''
                    parent_def_path = target_func
                    src_pq = []
                    has_sample = set()
                    for idx, (ty, primitive) in enumerate(tys):
                        prompt = ''
                        local_src = []
                        if ty is not None:
                            def_path = ty
                            if ty in type_to_def_path:
                                def_path = type_to_def_path[ty]
                            if target_func in deps:
                                if ty in deps[target_func]:
                                    bounds = deps[target_func][ty]
                                    cans = []
                                    if ty in candidates[target_func]:
                                        cans = candidates[target_func][ty]
                                    prompt = prompt_with_bounds(parent_def_path, def_path, ty, bounds, cans, deps, candidates,
                                                                data, crate, f, local_src, fd, set())
                            if len(prompt) == 0:
                                if is_std(ty):
                                    prompt = prompt_built_in(fd, parent_def_path, ty, file, crate)
                                else:
                                    prompt = prompt_with_src_only(parent_def_path, def_path, ty, data, fd, local_src, f, crate)
                            if prompt[0]:
                                has_sample.add(idx)
                                final_prompt += "For {}th argument, `{}` can be used, please use following sample code to construct it:\n```rust\n{}\n```\n".format(
                                    idx + 1, prompt[3], prompt[1])
                            else:
                                src_pq.extend(local_src)
                                final_prompt += "For {}th argument, `{}` can be used, please use following description to construct it:\n```\n{}\n```\n".format(
                                    idx + 1, prompt[3], prompt[2])
                        else:
                            final_prompt += "For {}th argument, its type is `{}`, please use some sample data to initialize it.\n".format(
                                idx + 1, primitive)
                    # request for unit test
                    target = target_func
                    deps = dependencies[target]
                    func_src = srcs[target][0]
                    trait_stmt = ''
                    if len(optional_trait) > 0:
                        trait_stmt = ", as an implmeent of `{}` trait".format(optional_trait)
                    pr_target = prompt_target.format(func_name, crate, file, target, trait_stmt, func_src)
                    src_pq = set(src_pq)
                    single_test_template = """ 
        #[cfg(test)]
        mod tests {{
            use super::*;
            {}
            #[test]
            fn test_rug() {{
                {}
                
                {}
            }}
        }}
                            """
                    succeed = False
                    for fc in reversed(func_call):
                        if '<' in fc and (' for ' in fc or ' as ' in fc):
                            continue
                        params = ''
                        param_template = "let mut p{} = ... ;\n"
                        no_sample = set()
                        for idx, (ty, primitive) in enumerate(tys):
                            params += param_template.format(idx)
                            if idx not in has_sample:
                                no_sample.add(idx)
                        option_t = ""
                        if len(optional_trait) > 0:
                            option_t = "use crate::{};".format(get_full_path(optional_trait))
                        tests = single_test_template.format(option_t, params, fc)
                        output = pr_target
                        test_template = """
        Please help me following steps on the code below to build the unit test:
        
        1. fill in the {} variables in the following code using the samples without modifications and keep the type declarations
        2. construct the variables {} based on hints if there isn't a sample and fill in the generic args if I didn't give you the generic args
        3. combine all the use statements and place them inside the `tests` mod, remove the duplicated use, but don't add new ones
        
        ```rust
        {}
        ```
                                """
                        output += test_template.format(", ".join(["p" + str(x) for x in has_sample]),
                                                       ", ".join(["p" + str(x) for x in no_sample]), tests)
                        output += final_prompt
                        for dep in src_pq:
                            code = ''
                            file_loc = ''
                            if dep in srcs:
                                code += srcs[dep][0]
                                file_loc = " in {}".format(get_real_path(srcs[dep][1]))
                            if dep in self_to_fn:
                                for c in self_to_fn[dep]:
                                    if c not in 'CloneCopyDebug':
                                        code += c + '\n'
                            if len(code) > 0:
                                output += prompt_struct.format(get_full_path(dep), file_loc, code)

                        print('=' * 40)
                        messages = copy.deepcopy(msgs)
                        messages.append({"role": "user", "content": output})
                        finished = False
                        count = 3
                        while not finished and count > 0:
                            has_ans, code = gpt_request(messages)
                            code = handle_gpt_output(code).replace("mod tests", "mod tests_rug_{}".format(uid))
                            if has_ans and compile_only(fd, file, code, crate):
                                finished = True
                                succeed = True
                            else:
                                count -= 1
                        if finished:
                            print('unit gen succeed', target_func)
                            ok += 1
                            break
                    if not succeed:
                        print('unit gen err', target_func)
    print(ok, exceed_16, total)


def run_single(fd):
    # print("python3.10 -u main.py {} {} > {}_{}.log 2>&1".format(fd, crate, fd, crate))
    subprocess.run("python3.10 -u main.py {} > {}/run.log 2>&1".format(fd, fd), shell=True)


if __name__ == '__main__':
    args = []
    if len(sys.argv) < 2:
        # os.chdir(sys.argv[1])
        for fd in os.listdir('.'):
            if not os.path.isdir(fd):
                continue
            # fd = sys.argv[1]
            fin = subprocess.run('cargo ws list -l', shell=True, cwd=fd, capture_output=True)
            if fin.returncode == 0:
                args.append(fd)
        # print(args)
        with multiprocessing.Pool(8) as p:
            p.map(run_single, args)
    else:
        fd = sys.argv[1]
        run_single_fd(fd)
