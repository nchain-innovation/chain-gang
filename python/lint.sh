#!/bin/bash

flake8 --ignore=E501,E131,E402,E722 src ../tools

mypy --check-untyped-defs --ignore-missing-imports src ../tools