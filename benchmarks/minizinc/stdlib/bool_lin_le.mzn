array[1..2] of var bool: bs;
constraint bool_lin_le([1, 1], bs, 1);
solve satisfy;
