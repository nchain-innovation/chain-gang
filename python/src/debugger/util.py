""" Utilities used by debugger
"""


def has_extension(fname: str, ext: str) -> bool:
    """ Return true if the file extension matches.
    """
    file_name = fname.split(".")
    if len(file_name) > 1:
        if file_name[-1] == ext:
            return True
    else:
        print(f"No file extension provided '{fname}'")
    return False
