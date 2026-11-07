pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body M8_Commerce_Webhook_Spark
  with SPARK_Mode => On
is
   function Verify_Payment_Webhook_Gate
     (HMAC_Matches       : Boolean;
      Webhook_Timestamp  : Unsigned_64;
      Server_Now_Unix    : Unsigned_64;
      Amount_Minor_Units : Unsigned_64;
      Requested_Caps     : Capability_Mask) return Webhook_Gate_Verdict
   is
      Skew : constant Unsigned_64 :=
        (if Webhook_Timestamp >= Server_Now_Unix then
           Webhook_Timestamp - Server_Now_Unix
         else
           Server_Now_Unix - Webhook_Timestamp);
   begin
      if not HMAC_Matches then
         return Webhook_Reject_Bad_HMAC;
      end if;

      if Skew > Max_Replay_Skew_Secs then
         return Webhook_Reject_Replay_Skew;
      end if;

      if Amount_Minor_Units = 0 then
         return Webhook_Reject_Zero_Amount;
      end if;

      if (Requested_Caps and Cap_Root_Seal) /= 0 then
         return Webhook_Reject_Charm18_Seal;
      end if;

      return Webhook_Verified_Mint;
   end Verify_Payment_Webhook_Gate;

   function Mint_ReadOnly_M8_Download_PTE
     (Gateway_Pole    : Balanced_Trit;
      Slot_ID         : Cap_Slot_Byte;
      Phys_Base_Block : Phys_Block_36;
      Blocks_Count    : Unsigned_32;
      Epoch_Gen       : Epoch_Gen_Byte) return Cap_PTE_Record
   is
      Aett_Base : constant Natural :=
        (case Gateway_Pole is
            when  1 => 0,
            when  0 => 9,
            when -1 => 18);
      Tag_27    : constant Futhark_Cube_Index :=
        Futhark_Cube_Index (Aett_Base + (Natural (Slot_ID) mod 9));
   begin
      return Cap_PTE_Record'(
        Phys_Base_Block => Phys_Base_Block,
        Max_Blocks      => Blocks_Count,
        Allowed_Caps    => Cap_Arena or Cap_M8,
        Expected_Epoch  => Epoch_Gen,
        Cap_Slot_ID     => Slot_ID,
        Futhark_Tag_R2  => Tag_27,
        Trit_Perm       => 0, -- Strictly Read-Only (`0`) for customer `.m8` downloads!
        Valid           => True);
   end Mint_ReadOnly_M8_Download_PTE;

end M8_Commerce_Webhook_Spark;
