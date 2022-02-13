# Private Insurance

## TODO description of project

## How to run protocol
Server receiving requests from UI can be run by
```
cargo run
```
In `priv-ins` folder.

There exists file `priv-ins/variable_config.json`. It contains an array of private inputs for nodes. Each input is a map from VariableId to Value. Example config is:
```
{
    "Nodes": [
        {
            "var_0_0": 42,
            "var_0_1": 43,
            "var_0_2": 44
        },
        {
            "var_1": 2137
        },
        {
            "var_2": 69420
        },
        {
            "var_3": 420
        },
        {
            "var_4_0": 1,
            "var_4_1": 2
        }
    ]
}
```
In this example 
* node number 0 has 3 variables var_0_0, var_0_1, var_0_2 with values 42, 43, 44.
* node number 1 has 1 variable var_1 with value 2137
* node number 2 has 1 variable var_2 with value 69420
* node number 3 has 1 variable var_3 with value 420
* node number 4 has 2 variables var_4_0, var_4_1 with values 1 and 2

Next complete a form for building building an arithmetic circuit in UI and submit request. Evaluated value should be printed in terminal by the server.

## How to run UI

## TODO UI running instructions
