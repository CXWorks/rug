import subprocess
import re
import os
import sys
import json
import tiktoken
import copy
import time
import multiprocessing
import openai.types.chat.chat_completion


init_content = """
You are an expert in Rust and I need your help on development. I will provide you the context and definition or sample 
and will ask you to help me write the code. Please pay attention to the paths and try to utilize the information I provided.
"""

def counter(fd:str):
    total = 0
    prompt = 0

    for f in os.listdir(fd):
        if f.endswith("_inject.log"):
            with open(fd+'/' +f, 'r') as fp:
                for l in fp.readlines():
                    if l.startswith('ChatCompletion(id='):
                        p = re.compile('completion_tokens=([0-9]+), prompt_tokens=([0-9]+), total_tokens=([0-9]+)')
                        gs = p.findall(l)[0]
                        if len(gs) != 3:
                            print(l, gs)
                        else:
                            prompt += int(gs[1])
                            total += int(gs[2])
    print(fd, prompt, total)


def calculate(fd:str):
    total = 0
    prompt = 0
    enc = tiktoken.encoding_for_model("gpt-4")
    if os.path.exists(fd+'/run.log'):
        with open(fd+'/run.log', 'r') as fp:
            ls = fp.readlines()
            idx = 0
            in_prompt = False
            ctxt = []
            while idx < len(ls):
                cl = ls[idx]
                if in_prompt:
                    if cl.startswith('--------------------'):
                        in_prompt = False
                        prompt += len(enc.encode(init_content +''.join(ctxt)))
                        ctxt = []
                else:
                    if cl.startswith('========================================'):
                        in_prompt = True
                        ctxt = []
                idx += 1
    print(fd, prompt)




if __name__ == '__main__':
    for f in os.listdir('.'):
        if os.path.isdir(f):
            calculate(f)

