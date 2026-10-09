// expect: 8:8: error: illegal cyclic type reference: upper bound U1 & U2
// expect: 1 error found
// The cycle check follows bounds, not the parts of one bound: a member named after seventeen
// others in an intersection is found as scalac finds it.
trait U1; trait U2; trait U3; trait U4; trait U5; trait U6; trait U7; trait U8; trait U9
trait U10; trait U11; trait U12; trait U13; trait U14; trait U15; trait U16; trait U17
trait T:
  type A <: U1 & U2 & U3 & U4 & U5 & U6 & U7 & U8 & U9 & U10 & U11 & U12 & U13 & U14 & U15 & U16 & U17 & A
