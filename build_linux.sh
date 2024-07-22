#!/bin/bash
cd obs-studio

rm -r build
cmake -S . -B build --preset ubuntu \
        -DCMAKE_BUILD_TYPE=RelWithDebInfo \
        -DENABLE_BROWSER:BOOL=OFF \
        -DENABLE_VLC:BOOL=OFF \
        -DENABLE_UI:BOOL=OFF \
        -DENABLE_VST:BOOL=OFF \
        -DENABLE_SCRIPTING:BOOL=OFF \
        -DCOPIED_DEPENDENCIES:BOOL=OFF \
        -DCOPY_DEPENDENCIES:BOOL=ON \
        -DBUILD_FOR_DISTRIBUTION:BOOL=ON \
        -DENABLE_WEBSOCKET:BOOL=OFF \
        -DCMAKE_COMPILE_WARNING_AS_ERROR=OFF

cmake --build build
cd build/rundir/RelWithDebInfo/lib/x86_64-linux-gnu
ln -s libobs.so.0 libobs.so
ln -s libobs-opengl.so.30 libobs-opengl.so
ln -s libobs-frontend-api.so.0 libobs-frontend-api.so