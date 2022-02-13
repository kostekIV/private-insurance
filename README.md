# Private Insurance

## Description of project

Private Insurance is the project about evaluating an insurance cost (and possibly value) as multi-party computation, where no one learns inputs of others. It is possible since both cost and value of insurance can be described as a function of private inputs of every party. We used `SPDZ` protocol with trusted dealer. In place of double-sharing we used hashing.

Project front-end was created in `Typescript` with `React` framework.
Project back-end was created in `Rust`.
Communication between uses `Tide` framework.
## How to run protocol
To run server receiving requests go to `priv-ins` folder and run it by
```
cargo run
```

To give values to variables used in protocol modify file `priv-ins/variable_config.json`. It contains an array of private inputs for nodes. Each input is a map from variable name to Value. Example config is:
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

Next complete a form for building an arithmetic circuit in UI and submit request. Evaluated value should be printed in terminal by the server.

## How to run UI

```
npm install
npm start
```
In `insur-front` folder.

Browser should authomatically open [http://localhost:3000](http://localhost:3000), which is UI page address. 

Fill all fields and press "submit" to send amount of parties and circut description for protocol.

### For proper operation both protocol and UI should run at the same time.
