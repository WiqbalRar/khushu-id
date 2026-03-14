cargo = find_program('cargo')

buildtype = get_option('buildtype')
cargo_options = []
target_subdir = 'debug'

if buildtype == 'release'
  cargo_options += ['--release']
  target_subdir = 'release'
endif

cargo_options_str = ' '.join(cargo_options)

cargo_build = custom_target('khushu-cargo-build',
  build_by_default: true,
  input: ['main.rs'],
  output: [meson.project_name()],
  console: true,
  install: true,
  install_dir: bindir,
  command: [
    'sh', '-c',
    'APP_ID=io.github.sniper1720.khushu GETTEXT_PACKAGE=khushu LOCALEDIR=@0@ VERSION=@1@ @2@ build @5@ --manifest-path @3@ && cp @4@/target/@6@/khushu @OUTPUT@'.format(
      localedir,
      meson.project_version(),
      cargo.full_path(),
      meson.project_source_root() + '/Cargo.toml',
      meson.project_source_root(),
      cargo_options_str,
      target_subdir
    )
  ]
)

glib_compile_resources = find_program('glib-compile-resources')
khushu_resources = custom_target('khushu-resources',
  input: '../data/khushu.gresource.xml',
  output: 'khushu-resources.gresource',
  command: [
    glib_compile_resources,
    '--target=@OUTPUT@',
    '--sourcedir=' + meson.current_source_dir() + '/../data',
    '@INPUT@'
  ],
  install: true,
  install_dir: join_paths(datadir, 'khushu')
)
