package lib

object Tests:
  inline def matchConstant(x: Any): String = x match
    case b: java.lang.constant.Constable => "hit"
    case _ => "next"

  inline def isConstant(x: Any): Boolean = x.isInstanceOf[java.lang.constant.Constable]

  inline def matchDescriptor(x: Any): String = x match
    case b: java.lang.constant.ConstantDesc => "hit"
    case _ => "next"

  inline def isDescriptor(x: Any): Boolean = x.isInstanceOf[java.lang.constant.ConstantDesc]
