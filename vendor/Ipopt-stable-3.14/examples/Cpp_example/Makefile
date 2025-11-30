# Copyright (C) 2003, 2010 International Business Machines and others.
# All Rights Reserved.
# This file is distributed under the Eclipse Public License.

##########################################################################
#    You can modify this example makefile to fit for your own program.   #
#    Usually, you only need to change the four CHANGEME entries below.   #
##########################################################################

# CHANGEME: This should be the name of your executable
EXE = cpp_example

# CHANGEME: Here is the name of all object files corresponding to the source
#           code that you wrote in order to define the problem statement
OBJS =  cpp_example.o \
	MyNLP.o

# CHANGEME: Additional libraries
ADDLIBS =

# CHANGEME: Additional flags for compilation (e.g., include flags)
ADDINCFLAGS =

##########################################################################
#  Usually, you don't have to change anything below.  Note that if you   #
#  change certain compiler options, you might have to recompile Ipopt.   #
##########################################################################

# C++ Compiler command
CXX = g++

# C++ Compiler options
CXXFLAGS = -O3 -fopenmp

# additional C++ Compiler options for linking
CXXLINKFLAGS =  -Wl,--rpath -Wl,/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib

prefix=/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local
exec_prefix=${prefix}

# Include directories
INCL = `PKG_CONFIG_PATH=/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig::/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig pkgconf --cflags ipopt` $(ADDINCFLAGS)
#INCL = -I${prefix}/include/coin-or  -I/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/include/coin-or/mumps  -DIPOPTLIB_BUILD $(ADDINCFLAGS)

# Linker flags
LIBS = `PKG_CONFIG_PATH=/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig::/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig pkgconf --libs ipopt`
#LIBS = -L${exec_prefix}/lib -lipopt  -L/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib -lcoinmumps -llapack -lblas -lm  -ldl

all: $(EXE)

.SUFFIXES: .cpp .o

$(EXE): $(OBJS)
	$(CXX) $(CXXLINKFLAGS) $(CXXFLAGS) -o $@ $(OBJS) $(LIBS) $(ADDLIBS)

clean:
	rm -rf $(EXE) $(OBJS)

.cpp.o:
	$(CXX) $(CXXFLAGS) $(INCL) -c -o $@ $<
