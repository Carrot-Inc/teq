// Two single-type imports of two classes under one name: the name is ambiguous where it is used (E049); the same
// class imported twice is none, and a type-on-demand import brings the type's member types.
// expect: J.java:4:32: error: reference to Date is ambiguous: it is both imported by name by import java.util.Date and imported by name subsequently by import java.sql.Date
object Use { def f = new p.J; def d = p.K.d(); def e = p.K.e() }
