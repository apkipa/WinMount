rem Build Core
pushd WinMountCore
rem cargo build
msbuild -t:Build -p:Configuration=Release -p:Platform=x64
popd

rem Build GUI
pushd WinMountGUI
msbuild -t:Restore -p:RestorePackagesConfig=true
msbuild -t:Build -p:Configuration=Release -p:Platform=x64
popd
