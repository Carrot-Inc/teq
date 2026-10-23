object O:
  def f =
    try 1 catch
      case _ => 2
        // tail