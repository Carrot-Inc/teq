// Diamonds of traits: each extends the two before it, so a class is reached by exponentially many paths. The view's
// walks over the class graph visit each class once a query: a linearization computed once (dotty's baseData cache), a
// base type's arguments once per class and arguments. The view of it ends in milliseconds; a walk that revisits would
// not end within the suite's bound on a view, and a view that fails is a failure no pending list takes. The generic
// diamond stops at 36, where scalac's own typing takes 17 s (at 60 it does not end in 200 s). The self types' diamond is
// diamond-self.
trait T0 { def f: Int = 1 }
trait T1 extends T0
trait T2 extends T1 with T0
trait T3 extends T2 with T1
trait T4 extends T3 with T2
trait T5 extends T4 with T3
trait T6 extends T5 with T4
trait T7 extends T6 with T5
trait T8 extends T7 with T6
trait T9 extends T8 with T7
trait T10 extends T9 with T8
trait T11 extends T10 with T9
trait T12 extends T11 with T10
trait T13 extends T12 with T11
trait T14 extends T13 with T12
trait T15 extends T14 with T13
trait T16 extends T15 with T14
trait T17 extends T16 with T15
trait T18 extends T17 with T16
trait T19 extends T18 with T17
trait T20 extends T19 with T18
trait T21 extends T20 with T19
trait T22 extends T21 with T20
trait T23 extends T22 with T21
trait T24 extends T23 with T22
trait T25 extends T24 with T23
trait T26 extends T25 with T24
trait T27 extends T26 with T25
trait T28 extends T27 with T26
trait T29 extends T28 with T27
trait T30 extends T29 with T28
trait T31 extends T30 with T29
trait T32 extends T31 with T30
trait T33 extends T32 with T31
trait T34 extends T33 with T32
trait T35 extends T34 with T33
trait T36 extends T35 with T34
trait T37 extends T36 with T35
trait T38 extends T37 with T36
trait T39 extends T38 with T37
trait T40 extends T39 with T38
trait T41 extends T40 with T39
trait T42 extends T41 with T40
trait T43 extends T42 with T41
trait T44 extends T43 with T42
trait T45 extends T44 with T43
trait T46 extends T45 with T44
trait T47 extends T46 with T45
trait T48 extends T47 with T46
trait T49 extends T48 with T47
trait T50 extends T49 with T48
trait T51 extends T50 with T49
trait T52 extends T51 with T50
trait T53 extends T52 with T51
trait T54 extends T53 with T52
trait T55 extends T54 with T53
trait T56 extends T55 with T54
trait T57 extends T56 with T55
trait T58 extends T57 with T56
trait T59 extends T58 with T57
trait T60 extends T59 with T58
class C extends T60 { val y = f }
trait G0[A] { def g(a: A): A = a }
trait G1[A] extends G0[A]
trait G2[A] extends G1[A] with G0[A]
trait G3[A] extends G2[A] with G1[A]
trait G4[A] extends G3[A] with G2[A]
trait G5[A] extends G4[A] with G3[A]
trait G6[A] extends G5[A] with G4[A]
trait G7[A] extends G6[A] with G5[A]
trait G8[A] extends G7[A] with G6[A]
trait G9[A] extends G8[A] with G7[A]
trait G10[A] extends G9[A] with G8[A]
trait G11[A] extends G10[A] with G9[A]
trait G12[A] extends G11[A] with G10[A]
trait G13[A] extends G12[A] with G11[A]
trait G14[A] extends G13[A] with G12[A]
trait G15[A] extends G14[A] with G13[A]
trait G16[A] extends G15[A] with G14[A]
trait G17[A] extends G16[A] with G15[A]
trait G18[A] extends G17[A] with G16[A]
trait G19[A] extends G18[A] with G17[A]
trait G20[A] extends G19[A] with G18[A]
trait G21[A] extends G20[A] with G19[A]
trait G22[A] extends G21[A] with G20[A]
trait G23[A] extends G22[A] with G21[A]
trait G24[A] extends G23[A] with G22[A]
trait G25[A] extends G24[A] with G23[A]
trait G26[A] extends G25[A] with G24[A]
trait G27[A] extends G26[A] with G25[A]
trait G28[A] extends G27[A] with G26[A]
trait G29[A] extends G28[A] with G27[A]
trait G30[A] extends G29[A] with G28[A]
trait G31[A] extends G30[A] with G29[A]
trait G32[A] extends G31[A] with G30[A]
trait G33[A] extends G32[A] with G31[A]
trait G34[A] extends G33[A] with G32[A]
trait G35[A] extends G34[A] with G33[A]
trait G36[A] extends G35[A] with G34[A]
class D extends G36[Int] { val z = g(1) }
