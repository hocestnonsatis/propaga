var 1..4: x;
constraint x <= 3;
solve :: restart_constant(64) :: int_search([x], input_order, indomain_reverse_split, complete) satisfy;
