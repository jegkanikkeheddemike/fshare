import type { ReactNode } from "react";

export const Disabled = (props: { disabled: boolean, children: ReactNode }) => {
    if (props.disabled) {
        return <div className="opacity-50 pointer-events-none">
            {props.children}
        </div>
    }
    return <div>
        {props.children}
    </div>
}