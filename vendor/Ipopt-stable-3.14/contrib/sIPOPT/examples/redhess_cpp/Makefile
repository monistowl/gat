# Copyright (C) 2010 Hans Pirnay
# All Rights Reserved.
# This file is distributed under the Eclipse Public License.

##########################################################################
#    You can modify this example makefile to fit for your own program.   #
#    Usually, you only need to change the four CHANGEME entries below.   #
##########################################################################

# CHANGEME: This should be the name of your executable
EXE = redhess_cpp

# CHANGEME: Here is the name of all object files corresponding to the source
#           code that you wrote in order to define the problem statement
OBJS =  redhess_cpp.o \
	MySensTNLP.o

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

# TODO there should be an sipopt .pc file from which to get flags

# Include directories
INCL = `PKG_CONFIG_PATH=/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig::/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig pkgconf --cflags ipopt` $(ADDINCFLAGS)
#INCL = -I${prefix}/include/coin  -I/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/include/coin-or/mumps  -DIPOPTLIB_BUILD $(ADDINCFLAGS)

# Linker flags
LIBS = `PKG_CONFIG_PATH=/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig::/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib/pkgconfig pkgconf --libs ipopt` -lsipopt
#LIBS = -L${exec_prefix}/lib -lsipopt -lipopt  -L/home/tom/Code/gat/.worktrees/ipopt-build/vendor/local/lib -lcoinmumps -llapack -lblas -lm  -ldl

all: $(EXE)

.SUFFIXES: .cpp .o

$(EXE): $(OBJS)
	$(CXX) $(CXXLINKFLAGS) $(CXXFLAGS) -o $@ $(OBJS) $(LIBS) $(ADDLIBS)

clean:
	rm -rf $(EXE) $(OBJS)

.cpp.o:
	$(CXX) $(CXXFLAGS) $(INCL) -c -o $@ $<
