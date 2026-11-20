// Two on-demand imports that both bring a class Date: the name is ambiguous (E049), as the reference finds it.
// expect: J.java:1:71: error: reference to Date is ambiguous: it is both imported by import java.util._ and imported subsequently by import java.sql._
object Use { def f = J.f() }
