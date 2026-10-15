; Functions remember the environment in which they were defined.
(function make-adder (x)
    (lambda (y) (+ x y)))
(set add-ten (make-adder 10))
(output "10 + 7 =" (add-ten 7))

; Each call has its own local variables.
(function factorial (n)
    (if (<= n 1) 1 (* n (factorial (- n 1)))))
(output "6! =" (factorial 6))
