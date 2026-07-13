var 1..3: a;
var 0..2: b;
var 1..3: c;
constraint c = min(a, b);
solve satisfy;
