var set of 1..2: s;
constraint card(s) = 1;
solve :: set_search([s], input_order, indomain_min, complete) satisfy;
