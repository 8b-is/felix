-- wire_frame.adb

package body Wire_Frame is

   procedure Ingest (F : in out Wire_C1_Frame; W : Rational) is
   begin
      F.Root_Weight := W;
   end Ingest;

end Wire_Frame;