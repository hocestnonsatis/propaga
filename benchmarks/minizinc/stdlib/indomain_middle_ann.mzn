var 1..5: x;
constraint x <= 4;
solve :: int_search([x], input_order, indomain_middle, complete) satisfy;
