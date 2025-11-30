# Copyright (C) 2006 International Business Machines and others.
# All Rights Reserved.
# This file is distributed under the Eclipse Public License.

##########################################################################
#    You can modify this example makefile to fit for your own program.   #
#    Usually, you only need to change CHANGEME entries below.            #
##########################################################################

# CHANGEME
# To compile other examples, either change the following line, or add the
# argument DRIVER=filename_without_extension to make, e.g.,
# `make DRIVER=hello'

DRIVER = driver

# CHANGEME
# This should be the name of your executable; change if you want a name
# that's different from the file name.

EXE = $(DRIVER)

# CHANGEME
# OBJS should include all object files necessary to build your program. For
# the examples, only one file is needed for each example. You will probably
# have more as your code grows.

OBJS =  $(DRIVER).o

# CHANGEME
# Additional libraries. The examples require only the COIN-OR libraries specified
# as LIBS below. You may need more.

ADDLIBS =

# CHANGEME
# Additional flags for compilation (e.g., include flags). As for libraries,
# the examples require only COIN-OR include files, specified as part of CXXFLAGS
# below.

ADDINCFLAGS =

# CHANGEME
# Directory to the sources for the (example) problem definition files. VPATH
# is used if you are building in a different directory than the source. This
# can be handy for various reasons; if none occur to you, don't worry about
# it.

SRCDIR = .


##########################################################################
#  Usually, you don't have to change anything below.  Note that if you   #
#  change certain compiler options, you might have to recompile the      #
#  package.                                                              #
##########################################################################

# C++ Compiler command
CXX = g++

# C++ Compiler options
CXXFLAGS = -O3

# C Compiler command
CC = gcc

# C Compiler options
CFLAGS = -O2 -DNDEBUG 

# Sample data directory
CXXFLAGS += -DSAMPLEDIR=\"`$(CYGPATH_W) /usr/share/coin/Data/Sample`\"

# Netlib data directory
#CXXFLAGS += -DNETLIBDIR=\"`$(CYGPATH_W) `\"

# additional C++ Compiler options for linking
CXXLINKFLAGS = 

# Include directories
INCL = `PKG_CONFIG_PATH=/home/tom/Code/gat/vendor/local/lib/pkgconfig::/home/tom/Code/gat/vendor/local/lib/pkgconfig pkgconf --cflags clp`
#INCL = -I${prefix}/include/coin-or
INCL += $(ADDINCFLAGS)

# Linker flags
LIBS = `PKG_CONFIG_PATH=/home/tom/Code/gat/vendor/local/lib/pkgconfig::/home/tom/Code/gat/vendor/local/lib/pkgconfig pkgconf --libs clp`
#LIBS = -L${exec_prefix}/lib -lClpSolver -lClp
LIBS += $(ADDLIBS)

# The following is necessary under cygwin, if native compilers are used
CYGPATH_W = echo

# get some directories, so we can expand ${exec_prefix}/lib and ${prefix}/include
prefix=/home/tom/Code/gat/vendor/local
exec_prefix=${prefix}

# Here we list all possible generated objects or executables to delete them
CLEANFILES = \
	addBits.o addBits \
	addColumns.o addColumns \
	addRows.o addRows \
	decomp2.o decomp2 \
	decomp3.o decomp3 \
	decompose.o decompose \
	defaults.o defaults \
	driver2.o driver2 \
	driver.o driver \
	driverC.o driverC \
	dualCuts.o dualCuts \
	ekk.o ekk \
	ekk_interface.o ekk_interface \
	hello.o hello \
	makeDual.o makeDual \
	minimum.o minimum \
	modify.o modify \
	network.o network \
	pdco.o pdco \
	piece.o piece \
	rowColumn.o rowColumn \
	sprint2.o sprint2 \
	sprint.o sprint \
	testBarrier.o testBarrier \
	testBasis.o testBasis \
	testGub2.o testGub2 \
	testGub.o testGub \
	testQP.o testQP \
	useVolume.o useVolume

all: $(EXE)

.SUFFIXES: .cpp .c .o

$(EXE): $(OBJS)
	$(CXX) $(CXXLINKFLAGS) $(CXXFLAGS) -o $@ $< $(LIBS)

clean:
	rm -rf $(CLEANFILES)

.cpp.o:
	$(CXX) $(CXXFLAGS) $(INCL) -c -o $@ `test -f '$<' || echo '$(SRCDIR)/'`$<

.c.o:
	$(CC) $(CFLAGS) $(INCL) -c -o $@ `test -f '$<' || echo '$(SRCDIR)/'`$<

