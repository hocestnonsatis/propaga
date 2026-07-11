var 3.0..4.0: x;
var 1.0..2.0: y;
var 3.0..8.0: z;
constraint x <= y;
constraint z = x * y;
solve satisfy;
