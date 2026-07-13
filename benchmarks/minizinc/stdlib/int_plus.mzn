var 1..5: a;
var 1..5: b;
var 2..10: c;
constraint c = a + b;
solve satisfy;
