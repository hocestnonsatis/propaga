var 1..5: x;
constraint x <= 4;
solve :: restart_none :: int_search([x], most_constrained, indomain_interval, complete) satisfy;
