array[1..2] of var bool: xs;
var bool: c;
constraint array_bool_xor(xs, c);
solve satisfy;
