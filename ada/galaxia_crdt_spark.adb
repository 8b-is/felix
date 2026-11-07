pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Exception_Propagation);

package body Galaxia_Crdt_Spark
  with SPARK_Mode => On
is
   function Ternary_Majority_3
     (T1, T2, T3 : Balanced_Trit) return Balanced_Trit
   is
      Sum_3 : constant Integer := Integer (T1) + Integer (T2) + Integer (T3);
   begin
      if T1 = T2 then
         return T1;
      elsif T1 = T3 then
         return T1;
      elsif T2 = T3 then
         return T2;
      elsif Sum_3 > 0 then
         return 1;
      elsif Sum_3 < 0 then
         return -1;
      else
         return 0;
      end if;
   end Ternary_Majority_3;

   procedure Verified_Byzantine_Gossip_Merge
     (Local_Stamp           : in out Peer_Epoch_Stamp;
      Local_Trit            : in out Balanced_Trit;
      Incoming_Magic        : in     M8_Magic_Bytes;
      Incoming_Poles        : in     Triadic_Seed_Poles;
      Incoming_Effects      : in     Capability_Mask;
      Incoming_Radius_Micro : in     Unsigned_32;
      Incoming_Seal_Valid   : in     Boolean;
      Wave_Coherence_Prod   : in     Integer_64;
      Incoming_Stamp        : in     Peer_Epoch_Stamp;
      Incoming_Trit         : in     Balanced_Trit;
      Accepted              : out    Boolean)
   is
      Header_OK : constant Boolean :=
        Verify_M8_Container_Header
          (Magic                 => Incoming_Magic,
           Poles                 => Incoming_Poles,
           Declared_Effects      => Incoming_Effects,
           Spectral_Radius_Micro => Incoming_Radius_Micro,
           Seal_Matches          => Incoming_Seal_Valid);
   begin
      if not Header_OK or else Wave_Coherence_Prod < 0 then
         Accepted := False;
         return;
      end if;

      if Stamp_Less_Or_Equal (Local_Stamp, Incoming_Stamp) then
         Local_Stamp := Incoming_Stamp;
         Local_Trit  := Incoming_Trit;
      end if;
      Accepted := True;
   end Verified_Byzantine_Gossip_Merge;

end Galaxia_Crdt_Spark;
