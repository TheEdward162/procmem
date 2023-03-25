import {
	ButtonItem,
	definePlugin,
	DialogButton,
	Menu,
	MenuItem,
	PanelSection,
	PanelSectionRow,
	Router,
	ServerAPI,
	showContextMenu,
	staticClasses,
} from "decky-frontend-lib";
import { FaMemory } from "react-icons/fa";
import { FC } from "react";

const Content: FC<{ serverAPI: ServerAPI }> = ({ }) => {
	return (
		<PanelSection title="Panel Section">
			<PanelSectionRow>
				<ButtonItem
					layout="below"
					onClick={(e: MouseEvent) =>
						showContextMenu(
							<Menu label="Menu" cancelText="CAAAANCEL" onCancel={() => { }}>
								<MenuItem onSelected={() => { }}>Item #1</MenuItem>
								<MenuItem onSelected={() => { }}>Item #2</MenuItem>
								<MenuItem onSelected={() => { }}>Item #3</MenuItem>
							</Menu>,
							e.currentTarget ?? window
						)
					}
				>
					Server says yolo
				</ButtonItem>
			</PanelSectionRow>

			<PanelSectionRow>
				<div style={{ display: "flex", justifyContent: "center" }}>
					<FaMemory />
				</div>
			</PanelSectionRow>

			<PanelSectionRow>
				<ButtonItem
					layout="below"
					onClick={() => {
						Router.CloseSideMenus();
						Router.Navigate("/decky-plugin-test");
					}}
				>
					Router
				</ButtonItem>
			</PanelSectionRow>
		</PanelSection>
	);
};

export default definePlugin((serverApi: ServerAPI) => {
	return {
		title: <div className={staticClasses.Title}>Procmem</div>,
		content: <Content serverAPI={serverApi} />,
		icon: <FaMemory />,
		onDismount() {

		}
	};
});