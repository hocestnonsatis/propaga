var 1..3: a;
var 5..9: b;
constraint a <= 2;
constraint b <= 8;
solve :: int_search([a, b], max_regret, indomain_min, complete) satisfy;
