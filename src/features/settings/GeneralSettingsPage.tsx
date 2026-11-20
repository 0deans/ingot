import DeveloperSettingsCard from "@/features/settings/components/DeveloperSettingsCard"
import IntegrationsNetworkCard from "@/features/settings/components/IntegrationsNetworkCard"
import LauncherBehaviorCard from "@/features/settings/components/LauncherBehaviorCard"
import UpdatesVersionCard from "@/features/settings/components/UpdatesVersionCard"

const GeneralSettingsPage = () => {
	return (
		<div className="flex flex-col gap-5">
			<LauncherBehaviorCard />
			<UpdatesVersionCard />
			<IntegrationsNetworkCard />
			<DeveloperSettingsCard />
		</div>
	)
}

export default GeneralSettingsPage
