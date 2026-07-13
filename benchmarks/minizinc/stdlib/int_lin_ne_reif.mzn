var 1..3: x;
var 1..3: y;
var bool: b;
constraint int_lin_ne_reif([1, 1], [x, y], 4, b);
solve satisfy;
