    from datasette.app import TEMPLATE_BASE_CONTEXT
    from datasette.template_contexts import PAGES
        title = "{} page".format(klass.__name__.removesuffix("Context"))