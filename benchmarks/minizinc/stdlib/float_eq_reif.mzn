var 1.0..3.0: x;
var 1.0..3.0: y;
var bool: b;
constraint float_eq_reif(x, y, b);
solve satisfy;
