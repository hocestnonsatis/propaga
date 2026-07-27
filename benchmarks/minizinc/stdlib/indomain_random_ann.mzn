var 1..3: x;
constraint x <= 2;
solve :: int_search([x], input_order, indomain_random, complete) satisfy;
