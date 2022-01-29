import React from "react";
import "./App.css";
import { useLazyExprQuery } from "./store/api/expression.service";
import { Expression } from "./model/expressions";

function App() {
  const hardcodedExpr: Expression = {
    binOp: {
      left: {
        number: {
          number: 10,
        },
      },
      right: {
        number: {
          number: 10,
        },
      },
      op: "Add",
    },
  };
  const [trigger, { isLoading, data, error }] = useLazyExprQuery();

  const onClick = () => {
    trigger(hardcodedExpr);
  };

  error && console.log(error);

  return (
    <div>
      {isLoading && <p>Loading</p>}
      {data && <p>Got response {data.msg}</p>}
      {error && <p>Error!</p>}
      <button onClick={onClick} >CLICK ME TO SUBMIT</button>
    </div>
  );
}

export default App;
