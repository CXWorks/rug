import subprocess
import os
import copy
import json
import sys


def read_targets():
    ret = subprocess.run('cargo clean && RUSTFLAGS="-C instrument-coverage -C debug-assertions=off" cargo test -- --list', shell=True, capture_output=True)
    bins = []
    if ret.returncode != 0:
        raise Exception("err read list")
    else:
        stderr = ret.stderr.decode('utf-8')
        for l in stderr.splitlines():
            if 'Running unittests' in l:
                st = l.find('(')
                ed = l.find(')')
                binary = l[st+1:ed]
                bins.append(binary)
    subprocess.run("rm *.profraw", shell=True)
    return bins


def execute_cov(bins: list):
    if os.path.exists('cov'):
        subprocess.run('rm -rf cov', shell=True, capture_output=True)
    os.mkdir('cov')
    HOME = os.getcwd() + '/'
    counter = 0

    for binary in bins:
        ret = subprocess.run('{} --list'.format(binary), shell=True, capture_output=True)
        for l in ret.stdout.decode('utf-8').splitlines():
            if 'tests_rug' in l or 'tests_llm_16' in l:
                tar = l.split(': ')[0]
                counter+=1
                subprocess.run('LLVM_PROFILE_FILE={}/cov/cov_{}.profraw timeout 2m {} --test {}'.format(HOME, str(counter), binary, tar), shell=True, capture_output=True)
                tar_fd = HOME.replace('_replay', '_fuzz')
                tars = tar.split('::')
                mod = ''
                fn = ''
                for i in range(len(tars)):
                    p = tars[i]
                    if 'tests_llm_16' in p or 'tests_rug_' in p:
                        mod = p
                        fn = tars[i+1]
                path = tar_fd+'/inputs/{}/{}/'.format(mod, fn)
                if os.path.exists(path):
                    counter+=1
                    # print('FUZZ_CORPUS={} LLVM_PROFILE_FILE={}/cov/cov_{}.profraw timeout 2m {} --test {}'.format(path, HOME, str(counter), binary, tar))
                    subprocess.run('FUZZ_CORPUS={} LLVM_PROFILE_FILE={}/cov/cov_{}.profraw timeout 2m {} --test {}'.format(path, HOME, str(counter), binary, tar), shell=True, capture_output=True)





def parse_file(file:str):
    ans = 1
    with open(file, 'r') as fp:
        for l in fp.readlines():
            if 'mod tests' in l or '#[cfg(test)]' in l:
                break
            ans += 1
    return ans


def collect_result(bins: list):
    fin = subprocess.run('~/snap/rustup/common/rustup/toolchains/nightly-2022-12-10-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata merge -sparse cov/* -o fuzz_cov.profdata', shell=True)
    assert fin.returncode == 0
    cmd = '~/snap/rustup/common/rustup/toolchains/nightly-2022-12-10-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-cov export --instr-profile=fuzz_cov.profdata '
    for binary in bins:
        cmd += "--object {} ".format(binary)
    fin = subprocess.run(cmd, shell=True, capture_output=True)
    assert fin.returncode == 0
    obj = json.loads(fin.stdout.decode('utf-8'))
    with open('cov.json', 'w') as fp:
        json.dump(obj, fp)
    map = {}
    reg_total = 0
    reg_count = 0
    line_total = 0
    line_count = 0
    func_total = 0
    func_count = 0
    HOME = os.getcwd() + '/'
    tar_fd = HOME.replace('_replay', '_cov')
    mamp_reg = {}
    if os.path.exists(tar_fd+'/'+'cov_map.json'):
        with open(tar_fd+'/'+'cov_map.json', 'r') as jp:
            mamp_reg = json.load(jp)

    for func in obj['data'][0]['functions']:
        assert len(func['filenames']) <= 1
        filename = func['filenames'][0]
        is_valid_func = True
        has_hit  = False
        if os.getcwd() in filename:
            # print(filename)
            if filename not in map:
                map[filename] = parse_file(filename)
            limit = map[filename]
            for regions in func['regions']:
                lst = regions[0]
                led = regions[2]
                count = regions[4]
                if led > limit:
                    is_valid_func = False
                    break
                k = filename.replace('_replay', '_cov') + '/' +str(lst) +'/' + str(regions[1]) +'/'+str(led) + '/'+ str(regions[3])
                if k in mamp_reg:
                    if count > 0:
                        mamp_reg[k] = 1

                else:
                    print('err not in ', k)
    with open('cov_map.json', 'w') as fp:
        json.dump(mamp_reg, fp)
    reg_total = len(mamp_reg)
    for k,v in mamp_reg.items():
        if v !=0 :
            reg_count+=1
    print(reg_count, reg_total)


def run_single(fd):
    # print("python3.10 -u main.py {} {} > {}_{}.log 2>&1".format(fd, crate, fd, crate))
    subprocess.run("python3.10 -u main.py {} > {}/cov.log 2>&1".format(fd, fd), shell=True)


if __name__ == '__main__':
    import multiprocessing
    if len(sys.argv) < 2:
        args = []
        for fd in os.listdir('.'):
            if not os.path.isdir(fd):
                continue
            args.append(fd)
        with multiprocessing.Pool(24) as p:
            p.map(run_single, args)
    else:
        os.chdir(sys.argv[1])
        binary = read_targets()
        execute_cov(binary)
        collect_result(binary)
