type BinaryOperator = "Add" | "Mul" | "Div" | "Sub";

type BinaryOp = {
  left: Expression;
  right: Expression;
  op: BinaryOperator;
};

type Variable = {
  name: string;
};

type Number = {
  number: number;
};

export type Expression =
  | { number: Number }
  | { binOp: BinaryOp }
  | { variable: Variable };