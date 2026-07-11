var bool: a;
var bool: b;
var bool: c;
constraint not a = b;
constraint a;
constraint b /\ b = c;
solve satisfy;
