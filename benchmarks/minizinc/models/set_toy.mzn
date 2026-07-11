var set of 1..5: a;
var set of 1..5: b;
var set of 1..5: c;
constraint card(a) = 2;
constraint a union b = c;
solve satisfy;
