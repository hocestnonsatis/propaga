var set of 1..3: a;
var set of 1..3: b;
var bool: r;
constraint set_eq_reif(a, b, r);
solve satisfy;
