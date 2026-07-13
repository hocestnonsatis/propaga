array[1..2] of var 1..3: left;
array[1..2] of var 1..3: right;
constraint lex_less(left, right);
solve satisfy;
