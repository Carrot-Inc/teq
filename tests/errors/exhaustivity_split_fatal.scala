// teq: --wconf any:error
// The exhaustivity warning of a match that splits 70 distinct sealed types is given (dotty's SpaceEngine has
// no bound on the splits), and as an error under `--wconf any:error`, as scalac's `-Werror` fails the build.
sealed trait T0
case class N0(x: T1) extends T0
case object Z0 extends T0
sealed trait T1
case class N1(x: T2) extends T1
case object Z1 extends T1
sealed trait T2
case class N2(x: T3) extends T2
case object Z2 extends T2
sealed trait T3
case class N3(x: T4) extends T3
case object Z3 extends T3
sealed trait T4
case class N4(x: T5) extends T4
case object Z4 extends T4
sealed trait T5
case class N5(x: T6) extends T5
case object Z5 extends T5
sealed trait T6
case class N6(x: T7) extends T6
case object Z6 extends T6
sealed trait T7
case class N7(x: T8) extends T7
case object Z7 extends T7
sealed trait T8
case class N8(x: T9) extends T8
case object Z8 extends T8
sealed trait T9
case class N9(x: T10) extends T9
case object Z9 extends T9
sealed trait T10
case class N10(x: T11) extends T10
case object Z10 extends T10
sealed trait T11
case class N11(x: T12) extends T11
case object Z11 extends T11
sealed trait T12
case class N12(x: T13) extends T12
case object Z12 extends T12
sealed trait T13
case class N13(x: T14) extends T13
case object Z13 extends T13
sealed trait T14
case class N14(x: T15) extends T14
case object Z14 extends T14
sealed trait T15
case class N15(x: T16) extends T15
case object Z15 extends T15
sealed trait T16
case class N16(x: T17) extends T16
case object Z16 extends T16
sealed trait T17
case class N17(x: T18) extends T17
case object Z17 extends T17
sealed trait T18
case class N18(x: T19) extends T18
case object Z18 extends T18
sealed trait T19
case class N19(x: T20) extends T19
case object Z19 extends T19
sealed trait T20
case class N20(x: T21) extends T20
case object Z20 extends T20
sealed trait T21
case class N21(x: T22) extends T21
case object Z21 extends T21
sealed trait T22
case class N22(x: T23) extends T22
case object Z22 extends T22
sealed trait T23
case class N23(x: T24) extends T23
case object Z23 extends T23
sealed trait T24
case class N24(x: T25) extends T24
case object Z24 extends T24
sealed trait T25
case class N25(x: T26) extends T25
case object Z25 extends T25
sealed trait T26
case class N26(x: T27) extends T26
case object Z26 extends T26
sealed trait T27
case class N27(x: T28) extends T27
case object Z27 extends T27
sealed trait T28
case class N28(x: T29) extends T28
case object Z28 extends T28
sealed trait T29
case class N29(x: T30) extends T29
case object Z29 extends T29
sealed trait T30
case class N30(x: T31) extends T30
case object Z30 extends T30
sealed trait T31
case class N31(x: T32) extends T31
case object Z31 extends T31
sealed trait T32
case class N32(x: T33) extends T32
case object Z32 extends T32
sealed trait T33
case class N33(x: T34) extends T33
case object Z33 extends T33
sealed trait T34
case class N34(x: T35) extends T34
case object Z34 extends T34
sealed trait T35
case class N35(x: T36) extends T35
case object Z35 extends T35
sealed trait T36
case class N36(x: T37) extends T36
case object Z36 extends T36
sealed trait T37
case class N37(x: T38) extends T37
case object Z37 extends T37
sealed trait T38
case class N38(x: T39) extends T38
case object Z38 extends T38
sealed trait T39
case class N39(x: T40) extends T39
case object Z39 extends T39
sealed trait T40
case class N40(x: T41) extends T40
case object Z40 extends T40
sealed trait T41
case class N41(x: T42) extends T41
case object Z41 extends T41
sealed trait T42
case class N42(x: T43) extends T42
case object Z42 extends T42
sealed trait T43
case class N43(x: T44) extends T43
case object Z43 extends T43
sealed trait T44
case class N44(x: T45) extends T44
case object Z44 extends T44
sealed trait T45
case class N45(x: T46) extends T45
case object Z45 extends T45
sealed trait T46
case class N46(x: T47) extends T46
case object Z46 extends T46
sealed trait T47
case class N47(x: T48) extends T47
case object Z47 extends T47
sealed trait T48
case class N48(x: T49) extends T48
case object Z48 extends T48
sealed trait T49
case class N49(x: T50) extends T49
case object Z49 extends T49
sealed trait T50
case class N50(x: T51) extends T50
case object Z50 extends T50
sealed trait T51
case class N51(x: T52) extends T51
case object Z51 extends T51
sealed trait T52
case class N52(x: T53) extends T52
case object Z52 extends T52
sealed trait T53
case class N53(x: T54) extends T53
case object Z53 extends T53
sealed trait T54
case class N54(x: T55) extends T54
case object Z54 extends T54
sealed trait T55
case class N55(x: T56) extends T55
case object Z55 extends T55
sealed trait T56
case class N56(x: T57) extends T56
case object Z56 extends T56
sealed trait T57
case class N57(x: T58) extends T57
case object Z57 extends T57
sealed trait T58
case class N58(x: T59) extends T58
case object Z58 extends T58
sealed trait T59
case class N59(x: T60) extends T59
case object Z59 extends T59
sealed trait T60
case class N60(x: T61) extends T60
case object Z60 extends T60
sealed trait T61
case class N61(x: T62) extends T61
case object Z61 extends T61
sealed trait T62
case class N62(x: T63) extends T62
case object Z62 extends T62
sealed trait T63
case class N63(x: T64) extends T63
case object Z63 extends T63
sealed trait T64
case class N64(x: T65) extends T64
case object Z64 extends T64
sealed trait T65
case class N65(x: T66) extends T65
case object Z65 extends T65
sealed trait T66
case class N66(x: T67) extends T66
case object Z66 extends T66
sealed trait T67
case class N67(x: T68) extends T67
case object Z67 extends T67
sealed trait T68
case class N68(x: T69) extends T68
case object Z68 extends T68
sealed trait T69
case class N69(x: T70) extends T69
case object Z69 extends T69
sealed trait T70
case object Z70 extends T70
def f(x: T0): Int = x match
  case N0(N1(N2(N3(N4(N5(N6(N7(N8(N9(N10(N11(N12(N13(N14(N15(N16(N17(N18(N19(N20(N21(N22(N23(N24(N25(N26(N27(N28(N29(N30(N31(N32(N33(N34(N35(N36(N37(N38(N39(N40(N41(N42(N43(N44(N45(N46(N47(N48(N49(N50(N51(N52(N53(N54(N55(N56(N57(N58(N59(N60(N61(N62(N63(N64(N65(N66(N67(N68(N69(Z70)))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))) => 1
// expect: exhaustivity_split_fatal.scala:216:21: error: match may not be exhaustive.
// expect: N63(Z64)
// expect: (More unmatched cases are elided)
// expect: It would fail on pattern case: N0(N1(N2(
