use crate::afxdp::socket::Action;

pub fn process_packet(_pkt: &[u8]) -> Action {
    #[cfg(all(feature = "tx"))]
    {
        return Action::Tx;
    }

    Action::Drop
}
