; Your course's Fibonacci example, unchanged apart from this comment.
(block
    (function fib (n)
        (if (< n 2)
            1
            (+ (fib (- n 1))
               (fib (- n 2))
            )
        )
    )
    (set x 0)
    (while (< x 10)
        (block
            (output x (fib x))
            (set x (+ x 1))
        )
    )
)
