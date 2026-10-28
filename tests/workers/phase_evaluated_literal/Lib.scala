inline def g(inline y: Int): Int = inline if y > 1 then 1 else 2
transparent inline def t(x: Int): Int = g(x)
