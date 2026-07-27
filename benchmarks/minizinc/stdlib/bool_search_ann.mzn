var bool: a;
var bool: b;
constraint bool2int(a) + bool2int(b) = 1;
solve :: restart_geometric(1.5, 32) :: bool_search([a, b], input_order, indomain_min, complete) satisfy;
