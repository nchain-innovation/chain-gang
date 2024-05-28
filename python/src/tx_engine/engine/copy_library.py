import os
import shutil


# decorator to ensure that a function is run only once
def run_once(f):
    def wrapper(*args, **kwargs):
        if not wrapper.has_run:  # type: ignore[attr-defined]
            wrapper.has_run = True  # type: ignore[attr-defined]
            return f(*args, **kwargs)
    wrapper.has_run = False  # type: ignore[attr-defined]
    return wrapper


def should_copy(src: str, dest: str):
    """ Returns true if should copy the file
    """
    if not os.path.isfile(dest):
        return True
    return os.path.getmtime(src) > os.path.getmtime(dest)


@run_once
def copy_library():
    """ Figure out current working directory (cwd) depth from the top of the project
        and copy the `.so` file to the cwd
    """
    cwd = os.getcwd().split('/')
    cwd.reverse()
    depth = cwd.index('chain-gang')
    # print(f"cwd={cwd}, depth={depth}")
    destination_file = "./chain_gang.so"
    source_file = (depth * "../") + "target/debug/libchain_gang.dylib"
    if should_copy(source_file, destination_file):
        print("Copy library...")
        shutil.copyfile(source_file, destination_file)


copy_library()
