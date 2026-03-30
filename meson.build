project('khushu', 'rust',
  version: '1.0.3',
  meson_version: '>= 1.2.0',
)

i18n = import('i18n')
gnome = import('gnome')

prefix = get_option('prefix')
bindir = join_paths(prefix, get_option('bindir'))
datadir = join_paths(prefix, get_option('datadir'))
localedir = join_paths(prefix, get_option('localedir'))

summary({
    'prefix': prefix,
    'bindir': bindir,
    'datadir': datadir,
    'localedir': localedir,
})

subdir('data')
subdir('po')
subdir('src')
