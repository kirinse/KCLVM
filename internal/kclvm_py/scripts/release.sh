#!/usr/bin/env bash

kclvm_source_dir="$topdir/internal/kclvm_py"
kclvm_install_dir="$topdir/_build/dist/$os/kclvm"

kclvm_release_file="kclvm-$os-latest.tar.gz"
kclvm_release_path="$topdir/_build"
kclvm_package_dir="$topdir/_build/dist/$os"

# rm site-packages
if [ -d "$kclvm_install_dir/lib/site-packages" ]; then
    rm -rf $kclvm_install_dir/lib/site-packages
fi

# copy kclvm
if [ -d $kclvm_install_dir/lib/site-packages/kclvm ]; then
   rm -rf $kclvm_install_dir/lib/site-packages/kclvm
fi

mkdir -p $kclvm_install_dir/lib/site-packages
cp -r $kclvm_source_dir $kclvm_install_dir/lib/site-packages/
mv $kclvm_install_dir/lib/site-packages/kclvm_py $kclvm_install_dir/lib/site-packages/kclvm

# rm __pycache__
find $kclvm_install_dir/lib/site-packages | grep -E "(/__pycache__$|\.pyc$|\.pyo$)" | xargs rm -rf

cd $kclvm_package_dir
tar -czvf $kclvm_release_file $kclvm_install_dir

# Print the summary.
echo "================ Summary ================"
echo "  $kclvm_release_path/$kclvm_release_file has been created"
