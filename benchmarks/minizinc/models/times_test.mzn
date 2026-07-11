var 1..5: x;
var 1..5: y;
var 1..25: z;
constraint z = x * y;
constraint x = 3;
constraint y = 4;
solve satisfy;
