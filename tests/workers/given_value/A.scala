trait Greeter { def hi: String }
given Greeter with { def hi = "hi" }
