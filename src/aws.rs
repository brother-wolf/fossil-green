use ::rusoto_ce::CostExplorerClient;
use ::rusoto_core::{Region, CredentialsError, HttpClient};
use ::rusoto_credential::{ChainProvider, ProfileProvider};

pub fn get_client(aws_profile: &str) -> Option<CostExplorerClient> {

    if aws_profile.is_empty() {
        ChainProvider::new();
        Some(CostExplorerClient::new(Region::UsEast1))
    } else {
        let _profile_provider: Result<ProfileProvider,CredentialsError> = ProfileProvider::new();
        match _profile_provider {
            Ok(mut _prov) => {
                _prov.set_profile(aws_profile);
                Some(CostExplorerClient::new_with(HttpClient::new().expect("failed to create request dispatcher"), _prov, Region::UsEast1))
            },
            Err(_e) => {
                println!("error in getting profile provider: {:?}", _e);
                None
            },
        }
    }
}
