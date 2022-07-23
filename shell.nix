{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    clang
    shaderc

    vulkan-headers
    vulkan-tools
    vulkan-tools-lunarg

    vulkan-loader
    vulkan-validation-layers
    vulkan-extension-layer

    xorg.libX11
    renderdoc
  ];

  buildInputs = with pkgs; [ rust-analyzer ];
  
  LD_LIBRARY_PATH = with pkgs.xorg;
    "${libX11}/lib:${libXcursor}/lib:${libXrandr}/lib:${libXi}/lib";
  SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";
}
