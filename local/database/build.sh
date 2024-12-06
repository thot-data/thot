#!/bin/bash

root=../..
releases=${root}/target/release
target=$(rustc -Vv | grep host | cut -f2 -d' ')
target_file=syre-local-database-${target}
target_out=${releases}/${target_file}

cargo build --release -F server
mkdir -p ${releases}
# mv ${releases}/syre-local-database ${target_out}
mv target/release/syre-local-database ${target_out}

# copy to other packages
lang=${root}/lang
python_path=${lang}/python/src/syre/bin
r_path=${lang}/r/inst

# create python bin path if it does not exist
mkdir -p ${python_path}

cp ${target_out} ${python_path}
cp ${target_out} ${r_path}

chmod a+x ${python_path}/${target_file} ${r_path}/${target_file}
