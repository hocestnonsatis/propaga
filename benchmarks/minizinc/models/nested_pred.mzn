predicate p(var int: a, var int: b) = a = b;
predicate q(var int: x, var int: y) = p(x, y);
var 1..3: u;
var 1..3: v;
constraint q(u, v);
constraint u = 2;
solve satisfy;
