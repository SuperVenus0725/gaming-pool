# Testing Engine

## Setup

1. Make sure python is installed
2. Replace LocalTerra/cofig/config.toml with the `config.toml` from this directory and then `docker-compose up`
3. Run `pip install -r requirements.txt`
4. Now To Run the Tests `python main.py`

## Notes:

Run and update the json from Astroport for the local_terra.json before running the tests.

Also ensure if `debug` is set to `True` localterra is running.

To Create A Virtual env for the script run [ For Windows P.S ]

        python -m pip install --upgrade pip
        python -m virtualenv venv 

and the to activate it :

        -cd venv
        -cd Scripts
        -activate.bat

And to deactivate

        -deactivate.bat

## Author

- [Utkarsh Varma](https://github.com/UvRoxx)