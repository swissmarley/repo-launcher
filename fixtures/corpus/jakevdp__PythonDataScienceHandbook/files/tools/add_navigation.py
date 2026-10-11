import os
import itertools
from ipykernel import kernelspec as ks
import nbformat
from nbformat.v4.nbbase import new_markdown_cell
from generate_contents import NOTEBOOK_DIR, REG, iter_notebooks, get_notebook_title
if __name__ == '__main__':