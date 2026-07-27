var set of 1..3: s;
constraint card(s) = 1;
solve :: set_search([s], input_order, indomain_max, complete) satisfy;
