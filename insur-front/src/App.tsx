import React, { useReducer } from "react";
import "./App.css";
import { useLazyExprQuery } from "./store/api/expression.service";

const formReducer = (state: any, event: any) => {
  return {
    ...state,
    [event.name]: event.value,
  };
};

function App() {
  const [formData, setFormData] = useReducer(formReducer, {
    amount_of_people: 0,
    expression: "Number",
  });
  const [trigger, { isLoading, data, error }] = useLazyExprQuery();

  const onClick = () => {
    trigger(formData);
  };

  error && console.log(error);

  const handleSubmit = (event: any) => {
    event.preventDefault();
    onClick();
  };

  const handleChange = (event: any) => {
    setFormData({
      name: event.target.name,
      value: event.target.value,
    });
  };

  const getValue = (name: string) => {
    let result = Object.entries(formData).filter(([key, _]) => {
      return key === name;
    });
    if (result.length === 0) {
      return;
    } else {
      return result[0][1] as string;
    }
  };

  const renderExpression = (type: string, name: string) => {
    switch (type) {
      case "Number":
        let num_name = name + "/number";
        return (
          <input
            name={num_name}
            step="1"
            type="number"
            onChange={handleChange}
            value={getValue(num_name)}
            placeholder="Enter number"
          />
        );
      case "Variable":
        let var_name = name + "/variable";
        return (
          <input
            name={var_name + "/var"}
            onChange={handleChange}
            value={getValue(var_name + "/var")}
            placeholder="Enter variable name"
          />
        );
      case "Expression":
        let left_name = name + "/left";
        let right_name = name + "/right";
        let op_name = name + "/op";

        return (
          <div>
            <select
              name={left_name}
              onChange={handleChange}
              value={getValue(left_name)}
            >
              <option value="Unknown">--Choose Type--</option>
              <option value="Number">Number</option>
              <option value="Variable">Variable</option>
              <option value="Expression">Expression</option>
            </select>
            {renderExpression(getValue(left_name) || "Unknown", name + "/left")}
            <br></br>
            <select
              name={op_name}
              onChange={handleChange}
              value={getValue(op_name)}
            >
              <option value="Unknown">--Choose Operator--</option>
              <option value="Sum">Sum</option>
              <option value="Mul">Mul</option>
            </select>
            <br></br>
            <select
              name={right_name}
              onChange={handleChange}
              value={getValue(right_name)}
            >
              <option value="Unknown">--Choose Type--</option>
              <option value="Number">Number</option>
              <option value="Variable">Variable</option>
              <option value="Expression">Expression</option>
            </select>
            {renderExpression(
              getValue(right_name) || "Unknown",
              name + "/right"
            )}
          </div>
        );
      case "Unknown":
        return;
    }
  };

  return (
    <div className="wrapper">
      <h1>Private Insurence</h1>
      <form onSubmit={handleSubmit}>
        <fieldset>
          <label>
            <p>Amount of people</p>
            <input
              step="1"
              type="number"
              name="amount_of_people"
              min={0}
              onChange={handleChange}
              value={formData.amount_of_people}
              placeholder="Enter amount of people"
            />
          </label>
        </fieldset>
        <fieldset>
          <label>
            <p>Expression</p>
            <select
              name="expression"
              onChange={handleChange}
              value={formData.expression || "Number"}
            >
              <option value="Number">Number</option>
              <option value="Variable">Variable</option>
              <option value="Expression">Expression</option>
            </select>
            {renderExpression(formData.expression, "expression")}
          </label>
        </fieldset>
        <button type="submit"> Submit </button>
      </form>
      <div>
        {isLoading && <p>Loading</p>}
        {data && <p>Got response {data.msg}</p>}
        {error && <p>Error!</p>}
      </div>
    </div>
  );
}

export default App;
