type BinaryOperator = "Add" | "Mul" | "Div" | "Sub";

type BinaryOp = {
  readonly left: Expression;
  readonly right: Expression;
  readonly op: BinaryOperator;
};

type Number = {
  readonly number: number;
};

export type Expression =
  | { readonly number: Number }
  | { readonly binOp: BinaryOp };
