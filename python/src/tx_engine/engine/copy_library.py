import os
import shutil
from typing import Optional


# decorator to ensure that a function is run only once
def run_once(f):
    def wrapper(*args, **kwargs):
        if not wrapper.has_run:
            wrapper.has_run = True
            return f(*args, **kwargs)
    wrapper.has_run = False
    return wrapper


@run_once
def copy_library():
    """ Figure out our depth from the top of the project
        and copy the `.so` file to the current working directory
    """
    print("copy library")
    cwd = os.getcwd().split('/')
    cwd.reverse()
    depth = cwd.index('chain-gang')
    # print(f"cwd={cwd}, depth={depth}")
    source_file = (depth * "../") + "target/debug/libchain_gang.dylib"
    shutil.copyfile(source_file, "./chain_gang.so")


copy_library()
