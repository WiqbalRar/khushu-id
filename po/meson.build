i18n.gettext('khushu', preset: 'glib', args: ['--keyword=tr:1'])

if get_option('extra_translations')
  msgfmt = find_program('msgfmt')
  langs = khushu_langs.keys()

  foreach lang : langs
    foreach domain : ['gtk40', 'libadwaita']
      po_file = '@0@.@1@.po'.format(domain, lang)

      mo_target = custom_target(
        '@0@-@1@-mo'.format(domain, lang),
        input: po_file,
        output: '@0@-@1@.mo'.format(domain, lang),
        command: [msgfmt, '-o', '@OUTPUT@', '@INPUT@'],
        install: false,
        build_by_default: true,
      )

      if domain == 'gtk40'
        aliases = ['gtk40']
      else
        aliases = ['libadwaita']
      endif

      foreach alias : aliases
        meson.add_install_script(
          'sh', '-c',
          'mkdir -p "$DESTDIR@0@" && cp -v "@1@" "$DESTDIR@0@/@2@.mo"'.format(
             get_option('prefix') / get_option('datadir') / 'khushu' / 'locale' / lang / 'LC_MESSAGES',
             mo_target.full_path(),
             alias
          )
        )
      endforeach
    endforeach
  endforeach
endif
