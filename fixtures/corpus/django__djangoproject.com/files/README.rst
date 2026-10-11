To run locally, you can either:
- Install and run from a virtual environment
Install and run locally from a virtual environment
#. Create a `Python 3.12 virtualenv and activate it <https://docs.python.org/3/library/venv.html>`_
    python3 -m pip install -r requirements/dev.txt
   you're on macOS and using bash) file and then run ``source ~/.bashrc`` (or
    python -m manage migrate
    python -m manage createsuperuser
    python -m manage loaddata dev_sites
    python -m manage loaddata doc_releases
    python -m manage update_docs
    python -m manage loaddata dashboard_production_metrics
    python -m manage loaddata dashboard_example_data
    python -m manage update_metrics
#. Finally, run the server::
Behind the scenes, this will run the usual ``python -m manage test`` management
    python -m coverage report
    python -m coverage html
**(Optional)** In case you're using an own virtualenv you can also run the
    python -m pip install -r requirements/tests.txt
    python -m manage test [list of app labels]
To also run the end-to-end tests, install the Playwright browsers::
    python -m playwright install --with-deps chromium firefox
Alternatively, you can also run the following command in a separate shell
to run all process at once:
If you just want to run one of the processes defined above use the
    foreman run web
That'll just run the www server.
When running ``python -m manage update_docs`` to build all documents it will
    python -m manage dumpdata dashboard --exclude dashboard.Datum --indent=4 > dashboard_production_metrics.json
    python -m manage makemessages -l en
    python -m manage compilemessages
    python -m manage test
Next, run a webhooks forwarder that relays Stripe's test-mode events to your
    docker compose run --rm web python -m manage test
    docker compose run --rm web make reset-local-db
   If preferred, refer to the "Install and run locally from a virtual environment"
    docker compose exec -it web python -m manage update_docs
    `pre-commit <https://pre-commit.com>`_ is a framework to run hooks written in many languages, and it manages the language toolchain and dependencies for running the hooks.
``prek run`` command: https://prek.j178.dev/cli/#prek-run If an error is found, the tool will go ahead