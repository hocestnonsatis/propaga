var 1..2: x;
var 1..2: y;
constraint x != y;
solve :: seq_search([
  int_search([x], input_order, indomain_min, complete),
  int_search([y], input_order, indomain_max, complete)
]) satisfy;
