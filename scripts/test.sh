#!/bin/bash

parallel "sleep {}; echo done {}" ::: 1 2 3 4