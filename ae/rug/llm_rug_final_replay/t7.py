import multiprocessing
import os
import subprocess
import json
import sys
import multiprocessing


def load(path):
    a = {}
    with open(path, 'r') as fp:
        a = json.load(fp)
    return a


def run_single(args):
    fd1 = args[0]
    fd2=args[1]
    tar = args[2]
    rp = '/fuzz/llm_base_cov/'
    rs = '/rug/llm_rug_final_cov/'
    if not os.path.exists(fd1 + tar+'/cov_map.json') or not os.path.exists(fd2 + tar+'/cov_map.json'):
        print('missing', tar)
        return 
    ma = load(fd1 + tar+'/cov_map.json')
    mb = load(fd2+tar+'/cov_map.json')
    ct = 0
    for k,v in ma.items():
        nk = k.replace(rp, rs)
        if nk in mb:
            if v != 0:
                mb[nk] = 1
        else:
            print('missing', k, nk, v)
            ct += 1
    total = len(mb)
    cover = 0
    for k,v in mb.items():
        if v !=0:
            cover += 1
    print(tar, cover, total, ct)

if __name__ == '__main__':
    args = []
    for f in os.listdir('.'):
        if os.path.isdir(f):
            run_single((sys.argv[1], sys.argv[2], f))

